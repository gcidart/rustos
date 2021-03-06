// FIXME: Make me pass! Diff budget: 25 lines.

#[derive(Debug)]
enum Duration {
    MilliSeconds(u64),
    Seconds(u32),
    Minutes(u16)
}

// What traits does `Duration` need to implement?
impl PartialEq for Duration {
    fn eq(&self, other: &Self) -> bool {
        use Duration::*;
        match (self, other) {
            (MilliSeconds(ref a), Seconds(ref b)) => *a==(*b as u64)*1000,
            (MilliSeconds(ref a), Minutes(ref b)) => *a==(*b as u64)*1000*60,
            (Seconds( a), Minutes(b) ) => *a==(*b as u32)*60,
            _   => false
        }
    }
}

#[test]

fn traits() {
    assert_eq!(Seconds(120), Minutes(2));
    assert_eq!(Seconds(420), Minutes(7));
    assert_eq!(MilliSeconds(420000), Minutes(7));
    assert_eq!(MilliSeconds(43000), Seconds(43));
}
