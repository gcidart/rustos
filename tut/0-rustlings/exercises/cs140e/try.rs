// FIXME: Make me compile. Diff budget: 12 line additions and 2 characters.



#[derive(Debug, Clone)]
struct ErrorA;
#[derive(Debug, Clone)]
struct ErrorB;

#[derive(Debug, Clone)]
enum Error {
    A(ErrorA),
    B(ErrorB),
}

// What traits does `Error` need to implement?
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "error")
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error +'static)> {
        None
    }
}

fn do_a() -> Result<u16, ErrorA> {
    Err(ErrorA)
}

fn do_b() -> Result<u32, ErrorB> {
    Err(ErrorB)
}

fn do_both() -> Result<(u16, u32), Error> {
    Ok((do_a().unwrap(), do_b().unwrap()))
}

fn main() {}
