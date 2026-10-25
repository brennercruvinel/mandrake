import unittest

from fizz import fizzbuzz


class T(unittest.TestCase):
    def test_all(self):
        self.assertEqual(fizzbuzz(9), "Fizz")
        self.assertEqual(fizzbuzz(10), "Buzz")
        self.assertEqual(fizzbuzz(30), "FizzBuzz")
        self.assertEqual(fizzbuzz(7), "7")


if __name__ == "__main__":
    unittest.main()
