import unittest

from slug import slugify


class H(unittest.TestCase):
    def test_cases(self):
        self.assertEqual(slugify("  Olá, Mundo! 2026 "), "ola-mundo-2026")
        self.assertEqual(slugify("Ação"), "acao")
        self.assertEqual(slugify("a--b__c"), "a-b-c")
        self.assertEqual(slugify("!!!"), "")
