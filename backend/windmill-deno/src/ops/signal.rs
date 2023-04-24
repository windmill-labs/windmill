// This is a sort of no-op polyfill for https://github.com/HurricanKai/deno/blob/3c9771deb2d615c47a2570023039c6a71f1c774b/runtime/ops/signal.rs
// We do not want to expose process signals to children, but in case they (or a package they depend on) uses signals, this will work (although, of course, never resolve).

use std::{borrow::Cow, cell::RefCell, rc::Rc};

use deno_core::{error::AnyError, op, OpState, Resource, ResourceId};

deno_core::extension!(
    deno_signal,
    ops = [op_signal_bind, op_signal_unbind, op_signal_poll],
    customizer = |ext: &mut deno_core::ExtensionBuilder| {
        ext.force_op_registration();
    },
);

// It's likely unecessary to register fake resources, but to keep signatures 1:1 the same, we do this.
struct FakeSignalResource;

impl Resource for FakeSignalResource {
    fn name(&self) -> Cow<str> {
        "signal".into()
    }

    fn close(self: Rc<Self>) {}
}

#[op]
fn op_signal_bind(state: &mut OpState, _sig: &str) -> Result<ResourceId, AnyError> {
    let rid = state.resource_table.add(FakeSignalResource);
    Ok(rid)
}

#[op]
pub fn op_signal_unbind(state: &mut OpState, rid: ResourceId) -> Result<(), AnyError> {
    state.resource_table.close(rid)?;
    Ok(())
}

#[op]
async fn op_signal_poll(_state: Rc<RefCell<OpState>>, _rid: ResourceId) -> Result<bool, AnyError> {
    futures::future::pending().await
}
