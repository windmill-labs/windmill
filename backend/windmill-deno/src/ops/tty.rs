use deno_core::{error::AnyError, op, OpState};

deno_core::extension!(
    deno_tty,
    ops = [op_stdin_set_raw, op_isatty, op_console_size],
    customizer = |ext: &mut deno_core::ExtensionBuilder| {
        ext.force_op_registration();
    },
);
#[op(fast)]
fn op_stdin_set_raw(state: &mut OpState, is_raw: bool, cbreak: bool) -> Result<(), AnyError> {
    Ok(())
}

#[op(fast)]
fn op_isatty(state: &mut OpState, rid: u32, out: &mut [u8]) -> Result<(), AnyError> {
    out[0] = 0; // false

    Ok(())
}

#[op(fast)]
fn op_console_size(state: &mut OpState, result: &mut [u32]) -> Result<(), AnyError> {
    let cols = 10;
    let rows = 10;

    result[0] = cols;
    result[1] = rows;

    Ok(())
}
