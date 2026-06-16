//! Wasm instance — bridge between the engine and the async executor.
//!
//! Performance contract: an `Instance` borrows its `&'static Module`
//! from the engine table — no Arc clone, no refcount touch on spawn.
//! The engine owns module lifetime; instances merely reference it.
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use super::engine::{engine, ModuleId};
use super::module::Module;
use super::store::Store;
use super::WasmError;
use crate::memory::linear_mem;
use crate::sched::executor::Executor;
use crate::sched::priority::Priority;
use crate::sched::task::Task;
use crate::wasi::ctx::WasiCtx;

pub struct Instance {
    pub module: &'static Module,
    pub store:  Store,
    pub state:  ExecState,
}

#[derive(Copy, Clone, Debug)]
pub enum ExecState {
    Cold, Running, AwaitingHost, Finished,
    Trapped(super::trap::Trap),
}

impl Instance {
    pub fn new(module: &'static Module, wasi: WasiCtx) -> Result<Self, WasmError> {
        let linear = linear_mem::allocate(module.initial_mem)
            .ok_or(WasmError::OutOfMemory)?;
        Ok(Self {
            module,
            store: Store::new(linear, wasi),
            state: ExecState::Cold,
        })
    }
}

impl Future for Instance {
    type Output = Result<(), WasmError>;
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.state {
            ExecState::Cold       => { self.state = ExecState::Running; Poll::Pending }
            ExecState::Running    => { self.state = ExecState::Finished; Poll::Ready(Ok(())) }
            ExecState::Finished   => Poll::Ready(Ok(())),
            ExecState::Trapped(t) => Poll::Ready(Err(WasmError::Trap(t))),
            ExecState::AwaitingHost => Poll::Pending,
        }
    }
}

/// Compile-and-spawn convenience.  Synchronous: no `.await` needed.
pub fn spawn(id: ModuleId) -> Result<(), WasmError> {
    let module = engine().module(id).ok_or(WasmError::Decode)?;
    let inst   = Instance::new(module, WasiCtx::default())?;
    Executor::global().spawn(Task::with_priority(async move {
        let _ = inst.await;
    }, Priority::Normal));
    Ok(())
}
