class Sample {
 int Outer(int a, int b, params int[] more) {
 System.Func<int,int> inner = x => { if (x > 0) { return x; } return 0; };
 if (a > b) { return inner(a); } return b;
} }
