use proc_macro::TokenStream;

fn make_handler(irq_no: u64) -> String {
    let func = format!(
        "extern \"x86-interrupt\" fn irq_handler_{irq_no} (_stack_frame: idt::StackFrame) {{ shared_handler({irq_no}, _stack_frame) }}",
    );

    func
}

#[proc_macro]
pub fn make_handlers(_item: TokenStream) -> TokenStream {
    let funcs: Vec<String> = (0..256 - 32).map(|i| make_handler(i)).collect();

    funcs.join("").parse().unwrap()
}

fn make_assignment(irq_no: u64) -> String {
    let assignment = format!(
        "idt.interrupts()[{irq_no}].set_handler_addr(irq_handler_{irq_no} as *const () as u64).options.set_present(true);"
    );

    assignment
}

#[proc_macro]
pub fn assign_handlers(_item: TokenStream) -> TokenStream {
    let assignments: Vec<String> = (0..256 - 32).map(|i| make_assignment(i)).collect();

    assignments.join("").parse().unwrap()
}
