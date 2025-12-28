F test_enumerate() {
    for(i, record) in self.history.iter().enumerate() {
        println!("Index: {}, Record: {}", i, record);
    }
}