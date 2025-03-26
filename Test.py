import sys
import unittest
from io import StringIO

import Lexer
from Parser.MainParser import Parser


class TestLanguage(unittest.TestCase):

    def setUp(self):
        self.held_output = StringIO()
        self.original_stdout = sys.stdout
        sys.stdout = self.held_output

    def tearDown(self):
        sys.stdout = self.original_stdout

    def test_declare_variable(self):
        code = "ni x = °X;"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = 10;")

    def test_declare_multiple_variables(self):
        code = "ni x = °X; ni y = °M;"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = 10;\nint y = 1000;")

    def test_arithmetic_operations(self):
        code = "ni x = °X + °X;"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = 10 + 10;")

    def test_assignment(self):
        code = "x = °X;"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "x = 10;")

    def test_assignment_with_variable(self):
        code = "ni x = °X; x = x + °X;"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = 10;\nx = x + 10;")

    def test_assignment_with_variable_and_function_call(self):
        code = "ni x = °X; x = add(°X, °X);"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = 10;\nx = add(10, 10);")

    def test_assignment_with_complex_expression(self):
        code = "ni x = °X + °X * °X;"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = 10 + 10 * 10;")

    def test_assignment_with_complex_expression_and_function_call(self):
        code = "ni x = add(°X, °X) * °X;"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = add(10, 10) * 10;")

    def test_assignment_with_complex_expression_and_function_call_and_variable(self):
        code = "ni x = add(°X, °X) * °X;"  # TODO: Make it work for more than one variable
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = add(10, 10) * 10;")

    def test_definition_function_call(self):
        code = "ni result = add(°X, °X);"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int result = add(10, 10);")

    def test_function_call(self):
        code = "add();"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "add();")

    def test_function_calls_with_params(self):
        code = "add(°X, °M);"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "add(10, 1000);")

    def test_addition_operation(self):
        code = "ni x = °X + °X;"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = 10 + 10;")

    def test_subtraction_operation(self):
        code = "ni x = °X - °X;"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = 10 - 10;")

    def test_multiplication_operation(self):
        code = "ni x = °X * °X;"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = 10 * 10;")

    def test_division_operation(self):
        code = "ni x = °X / °X;"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int x = 10 / 10;")

    def test_while_loop(self):
        code = "indem (°X maior_est °M) { ni x = x + °X; }"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "while (10 > 1000) {\nint x = x + 10;\n}")

    def test_while_loop_with_break(self):
        code = "indem (°X maior_est °M) { ni x = x + °X; abrumpe; }"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "while (10 > 1000) {\nint x = x + 10;\nbreak;\n}")

    def test_while_loop_with_continue(self):
        code = "indem (°X maior_est °M) { ni x = x + °X; continua; }"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "while (10 > 1000) {\nint x = x + 10;\ncontinue;\n}")

    def test_for_loop(self):
        code = "dum (ni i = °X; i maior_est °M; i = i + °X) { ni x = x + °X; }"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "for (int i = 10; i > 1000; i = i + 10) {\nint x = x + 10;\n}")

    # def test_complex_expressions(self):
    #     code = "ni x = (°A + °B) * °C;"
    #     tokens = Lexer.lexer(code)
    #     parser = Parser.Parser(tokens)
    #     result = parser.parse()
    #     self.assertEqual(result, "int x = (1 + 2) * 3;")
    #
    # def test_string_variables(self):
    #     code = "rts s = \"hello\";"
    #     tokens = Lexer.lexer(code)
    #     parser = Parser.Parser(tokens)
    #     result = parser.parse()
    #     self.assertEqual(result, "str s = \"hello\";")
    #
    def test_conditional_statement(self):
        code = "si (°X maior_est °X) { ni x = °I; }"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "if (10 > 10) {\nint x = 1;\n}")

    def test_if_else_statement(self):
        code = "si (°X maior_est °I) { ni x = °I; } aut { ni x = °X; }"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "if (10 > 1) {\nint x = 1;\n} else {\nint x = 10;\n}")

    def test_if_else_if_statement(self):
        code = "si (°X maior_est °I) { ni x = °I; } aut si (°X minor_est °I) { ni x = °X; }"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "if (10 > 1) {\nint x = 1;\n} else if (10 < 1) {\nint x = 10;\n}")

    def test_function_declaration(self):
        code = "munus ni add(ni x, ni y) { redde x + y; }"
        tokens = Lexer.lexer(code)
        parser = Parser(tokens, is_test=True)
        result = parser.parse()
        self.assertEqual(result, "int add(int x, int y) {\nreturn x + y;\n}")
