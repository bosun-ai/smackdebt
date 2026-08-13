class Worker {
  Worker(boolean ready) { if (ready) start(); }
  int run(int value) {
    java.util.function.IntUnaryOperator choose = item -> item > 0 ? item : 0;
    try {
      if (value > 1) return choose.applyAsInt(value);
      else if (value == 1) return 1;
      else return 0;
    } catch (RuntimeException error) { return -1; }
  }
}
