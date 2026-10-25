def moving_average(xs, k):
    """The mean of every window of k consecutive values, in order."""
    return [sum(xs[i : i + k]) / k for i in range(len(xs) - k + 1)]
