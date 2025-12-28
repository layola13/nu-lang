// Test case: Impl block with static method
S Operator {
    op: String,
}

I Operator {
    f from_str(s: String) -> Operator {
        l op = s;
        < Operator{op: op}
    }
}