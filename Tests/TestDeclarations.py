import sys
import unittest
from io import StringIO

import Lexer
from Parser.MainParser import Parser


class TestDeclarations(unittest.TestCase):

    def setUp(self):
        self.held_output = StringIO()
        self.original_stdout = sys.stdout
        sys.stdout = self.held_output

    def tearDown(self):
        sys.stdout = self.original_stdout

    def test_simple_declaration(self):
        code = "ni x;"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x;")

    def test_declaration_with_assignment(self):
        code = "ni x = °X;"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = 10;")

    def test_declaration_with_nullus(self):
        code = "ni x = nullus;"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = 0;")

    def test_multi_declaration_separate_lines(self):
        code = "ni x;\nni y;"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x;\nint y;")

    def test_declaration_with_complex_roman_numerals(self):
        code = "ni x = °MMXXI;"  # 2021
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = 2021;")

    def test_declaration_with_expression(self):
        code = "ni x = °X * °V + °II;"  # 10 * 5 + 2
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = 10 * 5 + 2;")

    def test_declaration_with_parentheses_expression(self):
        code = "ni x = (°X + °V) * °II;"  # (10 + 5) * 2
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = (10 + 5) * 2;")

    def test_declaration_with_function_call(self):
        code = "ni x = add(°V, °X);"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = add(5, 10);")

    def test_multiple_declarations_with_assignments(self):
        code = "ni x = °X; ni y = °V; ni z = x + y;"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = 10;\nint y = 5;\nint z = x + y;")

    def test_declaration_with_self_reference_error(self):
        # This should typically result in an error or special handling
        code = "ni x = x + °I;"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        # Check if appropriate error handling or result occurs
        # Actual behavior depends on your implementation
        self.assertIn("x = x + 1", result)

