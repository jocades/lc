fn main() {
    let mut stack = vec![1, 2, 3, 4];
    //                         ^ fun slots
    let slots: *mut i32 = &mut stack[2]; // *mut 3

    unsafe {
        let base = slots.offset_from(stack.as_ptr());
        dbg!(base);
        stack.truncate(base as usize);
        dbg!(stack);
    }
    // slots.offset_from()
}
