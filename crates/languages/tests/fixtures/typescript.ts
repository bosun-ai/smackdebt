class Worker {
  handler = (ready: boolean): number => ready ? 1 : 0;
  run(value: number): number {
    if (value > 0) return this.handler(true);
    return 0;
  }
}
