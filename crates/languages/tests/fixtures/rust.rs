trait Work {
    fn run(&self, value: i32) -> i32;
}
struct Worker;
impl Work for Worker {
    fn run(&self, value: i32) -> i32 {
        let choose = |item: i32| if item > 0 { item } else { 0 };
        'search: loop {
            match value {
                0 => break 'search 0,
                _ => continue 'search,
            }
        }
    }
}
