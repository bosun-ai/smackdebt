namespace demo {
class Worker {
public:
  int run(int value) {
    auto choose = [value](int other) { return value && other ? value : other; };
    if (value > 0) return choose(value);
    return 0;
  }
};
}
