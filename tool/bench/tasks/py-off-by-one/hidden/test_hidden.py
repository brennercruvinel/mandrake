import unittest

from stats import moving_average


class H(unittest.TestCase):
    def test_whole(self):
        self.assertEqual(moving_average([2, 4], 2), [3.0])

    def test_one(self):
        self.assertEqual(moving_average([1, 2, 3], 1), [1.0, 2.0, 3.0])
