// 控制流验收：while 累加 1..10（应得 55），并用两个 if/else 分别覆盖真/假分支。
// then_ran / else_ran 默认 1000：只要对应分支没被执行，结果就会偏离 55。
fn main() -> i32 {
    let sum = 0;
    let i = 1;
    while i <= 10 {
        sum = sum + i;
        i = i + 1;
    }
    let then_ran = 1000;
    if sum == 55 {
        then_ran = 0;
    }
    let else_ran = 1000;
    if sum == 0 {
        else_ran = 1000;
    } else {
        else_ran = 0;
    }
    sum + then_ran + else_ran
}