use proc_macro::TokenStream;

fn make_handler(irq_no: u64) -> String {
    let func = format!(
        "extern \"x86-interrupt\" fn irq_handler_{irq_no} (_stack_frame: idt::StackFrame) {{ shared_handler({irq_no}, _stack_frame) }}",
    );

    func
}

#[proc_macro]
pub fn make_handlers(_item: TokenStream) -> TokenStream {
    let funcs: Vec<String> = (32..256).map(make_handler).collect();

    funcs.join("").parse().unwrap()
}

fn make_assignment(irq_no: u64) -> String {
    let assignment = format!(
        "idt.interrupts()[{irq_no} - 32].set_handler_addr(irq_handler_{irq_no} as *const () as u64).options.set_present(true);"
    );

    assignment
}

#[proc_macro]
pub fn assign_handlers(_item: TokenStream) -> TokenStream {
    let assignments: Vec<String> = (32..256).map(make_assignment).collect();

    assignments.join("").parse().unwrap()
}
