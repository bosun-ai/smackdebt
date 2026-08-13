class Worker {
  handler = (ready) => ready && this.run(1);
  run(value) {
    if (value > 1) return value;
    else if (value === 1) return 1;
    return 0;
  }
}
