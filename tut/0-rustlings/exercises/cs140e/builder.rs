// FIXME: Make me pass! Diff budget: 30 lines.


#[derive(Debug,Default)]
struct Builder {
    string: Option<String>,
    number: Option<usize>,
}

impl Builder {
    // fn string(...
    fn string<S: Into<String>>(&mut self, s:S) -> &mut Self {
        self.string=Some(s.into());
        self
    }
    // fn number(...
    fn number(&mut self, x:usize) -> &mut Self {
        self.number= Some(x);
        self
    }
}

impl ToString for Builder {
    // Implement the trait
    fn to_string(&self) ->String {
        if self.string.is_some() && self.number.is_some() {
            format!("{} {}", self.string.as_ref().unwrap().to_string(), self.number.unwrap())
            //concat!(self.string.unwrap(), " ", self.number.unwrap())
        } else if self.string.is_some() {
            self.string.as_ref().unwrap().to_string()
        } else if self.number.is_some() {
            self.number.unwrap().to_string()
        } else {
            "".to_string()
        }
    }
}

// Do not modify this function.
#[test]
fn builder() {
    let empty = Builder::default().to_string();
    assert_eq!(empty, "");

    let just_str = Builder::default().string("hi").to_string();
    assert_eq!(just_str, "hi");

    let just_num = Builder::default().number(254).to_string();
    assert_eq!(just_num, "254");

    let a = Builder::default()
        .string("hello, world!")
        .number(200)
        .to_string();

    assert_eq!(a, "hello, world! 200");

    let b = Builder::default()
        .string("hello, world!")
        .number(200)
        .string("bye now!")
        .to_string();

    assert_eq!(b, "bye now! 200");

    let c = Builder::default()
        .string("heap!".to_owned())
        .to_string();

    assert_eq!(c, "heap!");
}
