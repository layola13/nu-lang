fn test_closure() -> i32 {
    let add = |x: i32, y: i32| -> i32 { x + y }
    let temp = 5
    let result = add(temp, 3)
    return result;
}
