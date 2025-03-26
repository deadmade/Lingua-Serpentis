import sys
import unittest
from io import StringIO

import Lexer
from Parser.MainParser import Parser


class TestPrinting(unittest.TestCase):

    def setUp(self):
        self.held_output = StringIO()
        self.original_stdout = sys.stdout
        sys.stdout = self.held_output

    def tearDown(self):
        sys.stdout = self.original_stdout

    def test_basic_print(self):
        code = "scribe(\"Hello, World!\");"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "printf(\"\"Hello, World!\"\");")

    def test_print_with_variable(self):
        code = "ni x = °X; scribe(x);"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = 10;\nprintf(\"%d\", x);")

    def test_print_with_expression(self):
        code = "scribe(°X + °V);"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "printf(\"%d\", 10 + 5);")

    def test_print_with_function_call(self):
        code = "scribe(add(°X, °V));"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "printf(\"%s\", add(10, 5));")

    def test_print_with_complex_expression(self):
        code = "scribe(°X * °V + (°II - °I));"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "printf(\"%d\", 10 * 5 + (2 - 1));")

    def test_print_missing_parenthesis(self):
        code = "scribe(\"Missing closing parenthesis\";"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, '')

    def test_print_empty_call(self):
        code = "scribe();"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "printf(\"\");")

    def test_multiple_print_statements(self):
        code = "scribe(\"First\"); scribe(\"Second\");"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "printf(\"\"First\"\");\nprintf(\"\"Second\"\");")

    def test_print_in_if_statement(self):
        code = "si (°X maior_est °V) { scribe(\"X is greater\"); }"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "if (10 > 5) {\nprintf(\"X is greater\");\n}")

    def test_print_nullus(self):
        code = "scribe(nullus);"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "printf(\"%d\", 0);")

    def test_print_with_string_concat(self):
        code = "scribe(\"Value: \" + °X);"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        # This may depend on your implementation
        self.assertTrue("printf" in result and "Value:" in result and "10" in result)

    # Not Supported
    # def test_print_format_strings(self):
    #     code = "ni x = °X; scribe(\"Value: %d\", x);"
    #     tokens = Lexer.lexer(code)
    #     parser = Parser(tokens, is_test=True)
    #     result = parser.parse()
    #     self.assertEqual(result, "int x = 10;\nprintf(\"Value: %d\", x);")
