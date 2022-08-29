use core::cell::RefMut;
use core::mem::size_of;

use lazy_static::lazy_static;

use crate::cell::Cell;
use crate::mm::PhysFrame;
use crate::trap::{CalleeRegs, SyscallFrame, TrapFrame};
use crate::user::{syscall_switch_to_user_space, trap_switch_to_user_space, UserSpace};
use crate::{prelude::*, UPSafeCell};

use super::processor::{current_task, schedule, PROCESSOR};
use super::scheduler::add_task;

core::arch::global_asm!(include_str!("switch.S"));
#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct TaskContext {
    pub regs: CalleeRegs,
    pub rip: usize,
}

extern "C" {
    pub fn context_switch(cur: *mut TaskContext, nxt: *const TaskContext);
}

pub fn context_switch_to_user_space() {
    let task = Task::current();
    let switch_space_task = SWITCH_TO_USER_SPACE_TASK.get();
    if task.inner_exclusive_access().is_from_trap {
        *switch_space_task.trap_frame() = *task.trap_frame();
        unsafe {
            trap_switch_to_user_space(
                &task.user_space.as_ref().unwrap().cpu_ctx,
                switch_space_task.trap_frame(),
            );
        }
    } else {
        *switch_space_task.syscall_frame() = *task.syscall_frame();
        unsafe {
            syscall_switch_to_user_space(
                &task.user_space.as_ref().unwrap().cpu_ctx,
                switch_space_task.syscall_frame(),
            );
        }
    }
}

lazy_static! {
    /// This variable is mean to switch to user space and then switch back in `UserMode.execute`
    ///
    /// When context switch to this task, there is no need to set the current task
    pub static ref SWITCH_TO_USER_SPACE_TASK : Cell<Task> = unsafe{
        Cell::new({
        let task = Task{
            func: Box::new(context_switch_to_user_space),
            data: Box::new(None::<u8>),
            user_space: None,
            task_inner: unsafe {
                UPSafeCell::new(TaskInner {
                    task_status: TaskStatus::Runnable,
                    ctx: TaskContext::default(),
                    is_from_trap:false,
                })
            },
            exit_code: usize::MAX,
            kstack: KernelStack::new(),
        };
        task.task_inner.exclusive_access().task_status = TaskStatus::Runnable;
        task.task_inner.exclusive_access().ctx.rip = context_switch_to_user_space as usize;
        task.task_inner.exclusive_access().ctx.regs.rsp = task.kstack.frame.end_pa().kvaddr().0
            as usize
            - size_of::<usize>()
            - size_of::<SyscallFrame>();
        task
    })};
}

pub struct KernelStack {
    frame: PhysFrame,
}

impl KernelStack {
    pub fn new() -> Self {
        Self {
            frame: PhysFrame::alloc().expect("out of memory"),
        }
    }
}

/// A task that executes a function to the end.
pub struct Task {
    func: Box<dyn Fn() + Send + Sync>,
    data: Box<dyn Any + Send + Sync>,
    user_space: Option<Arc<UserSpace>>,
    task_inner: UPSafeCell<TaskInner>,
    exit_code: usize,
    /// kernel stack, note that the top is SyscallFrame/TrapFrame
    kstack: KernelStack,
}

pub struct TaskInner {
    pub task_status: TaskStatus,
    pub ctx: TaskContext,
    /// whether the task from trap. If it is Trap, then you should use read TrapFrame instead of SyscallFrame
    pub is_from_trap: bool,
}

impl Task {
    /// Gets the current task.
    pub fn current() -> Arc<Task> {
        current_task().unwrap()
    }

    /// get inner
    pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskInner> {
        self.task_inner.exclusive_access()
    }

    /// get inner
    pub fn inner_ctx(&self) -> TaskContext {
        self.task_inner.exclusive_access().ctx
    }

    /// Yields execution so that another task may be scheduled.
    ///
    /// Note that this method cannot be simply named "yield" as the name is
    /// a Rust keyword.
    pub fn yield_now() {
        todo!()
    }

    /// Spawns a task that executes a function.
    ///
    /// Each task is associated with a per-task data and an optional user space.
    /// If having a user space, then the task can switch to the user space to
    /// execute user code. Multiple tasks can share a single user space.
    pub fn spawn<F, T>(
        task_fn: F,
        task_data: T,
        user_space: Option<Arc<UserSpace>>,
    ) -> Result<Arc<Self>>
    where
        F: Fn() + Send + Sync + 'static,
        T: Any + Send + Sync,
    {
        /// all task will entering this function
        /// this function is mean to executing the task_fn in Task
        fn kernel_task_entry() {
            let current_task = current_task()
                .expect("no current task, it should have current task in kernel task entry");
            current_task.func.call(())
        }
        let result = Self {
            func: Box::new(task_fn),
            data: Box::new(task_data),
            user_space,
            task_inner: unsafe {
                UPSafeCell::new(TaskInner {
                    task_status: TaskStatus::Runnable,
                    ctx: TaskContext::default(),
                    is_from_trap: false,
                })
            },
            exit_code: 0,
            kstack: KernelStack::new(),
        };

        result.task_inner.exclusive_access().task_status = TaskStatus::Runnable;
        result.task_inner.exclusive_access().ctx.rip = kernel_task_entry as usize;
        result.task_inner.exclusive_access().ctx.regs.rsp = result.kstack.frame.end_pa().kvaddr().0
            as usize
            - size_of::<usize>()
            - size_of::<SyscallFrame>();

        let arc_self = Arc::new(result);
        add_task(arc_self.clone());

        schedule();
        Ok(arc_self)
    }

    pub fn syscall_frame(&self) -> &mut SyscallFrame {
        unsafe {
            &mut *(self
                .kstack
                .frame
                .end_pa()
                .kvaddr()
                .get_mut::<SyscallFrame>() as *mut SyscallFrame)
                .sub(1)
        }
    }

    pub fn trap_frame(&self) -> &mut TrapFrame {
        unsafe {
            &mut *(self.kstack.frame.end_pa().kvaddr().get_mut::<TrapFrame>() as *mut TrapFrame)
                .sub(1)
        }
    }

    /// Returns the task status.
    pub fn status(&self) -> TaskStatus {
        self.task_inner.exclusive_access().task_status
    }

    /// Returns the task data.
    pub fn data(&self) -> &dyn Any {
        &self.data
    }

    /// Returns the user space of this task, if it has.
    pub fn user_space(&self) -> Option<&Arc<UserSpace>> {
        if self.user_space.is_some() {
            Some(self.user_space.as_ref().unwrap())
        } else {
            None
        }
    }

    pub fn exit(&self) -> ! {
        self.inner_exclusive_access().task_status = TaskStatus::Exited;
        schedule();
        unreachable!()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// The status of a task.
pub enum TaskStatus {
    /// The task is runnable.
    Runnable,
    /// The task is running.
    Running,
    /// The task is sleeping.
    Sleeping,
    /// The task has exited.
    Exited,
}
