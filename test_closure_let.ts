import { Result, Ok, Err, $unwrap, $fmt, isSome, isnull, $match } from './nu_runtime';



function test_closure(): number {
    const add = (x: number, y: number): number => x + y;
    const temp = 5;
    const result = add(temp, 3);
    return result;
}

