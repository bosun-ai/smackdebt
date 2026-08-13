int controls(int value, int *items, int count) {
    if (value > 0 && count > 0) value--;
    for (int i = 0; i < count; i++) { while (items[i]) items[i]--; }
    switch (value) { case 0: return 0; default: break; }
    return value ? value : count;
}
