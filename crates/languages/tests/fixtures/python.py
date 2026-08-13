def outer(items):
    def inner(value):
        try:
            if value > 1:
                return value
            elif value == 1:
                return 1
        except ValueError:
            return 0
        return [item for item in items if item]
    choose = lambda value: value if value else 0
    return inner(choose(1))
