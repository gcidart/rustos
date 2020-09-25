use core::panic::PanicInfo;
use crate::console::kprintln;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    match _info.location() {
        Some(location) => {
            kprintln!("            (");
            kprintln!("       (      )     )");
            kprintln!("         )   (    (");
            kprintln!("        (          `");
            kprintln!("    .-\"\"^\"\"\"^\"\"^\"\"\"^\"\"-.");
            kprintln!("(//\\//\\//\\//\\//\\//)");
            kprintln!("  ~\\^^^^^^^^^^^^^^^^^^/~");
            kprintln!("     `================`");
            kprintln!();
            kprintln!("    The pi is overdone.");
            kprintln!();
            kprintln!("---------- PANIC ----------");
            kprintln!();
            kprintln!();
            kprintln!("FILE: {}", location.file());
            kprintln!("LINE: {}", location.line());
            kprintln!("COL: {}", location.column());
        },
        None => kprintln!("panic occurred ")
    }
    loop {}
}
