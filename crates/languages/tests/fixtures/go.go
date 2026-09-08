package sample
func outer(a, b int, more ...int) int {
 inner := func(x int) int { if x > 0 { return x }; return 0 }
 if a > b { return inner(a) }; return b
}
