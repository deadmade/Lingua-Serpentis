# Example for Puuuuuul

TOKEN_SPECS = [
    ("IDENTIFIER", r"[a-zA-Z_]\w*"),
    ("ASSIGN", r"="),
    ("PLUS", r"\+"),
    ("MINUS", r"-"),
    ("MULT", r"\*"),
    ("DIV", r"/"),
    ("LPAREN", r"\("),
    ("RPAREN", r"\)"),
    ("NEWLINE", r"\n"),
    ("SKIP", r"[ \t]+"),  # Ignoriere Leerzeichen
    ("NUMBER", r"°(((X|M||V|I|L)+.?)*(X|M||V|I|L))|nullus"), # Römische Zahlen  ich  kann  halt kei regex bei fragen erklär ichs
    ("STRING", r"\".*?\""),  # Strings as type
    ("CHAR","lit"),#littera,
    ("String","voc"), #vocabulum
    ("IF", r"si"),
    ("ELSE", r"aut"),
    ("WHILE", r"indem"),
    ("FOR", r"dum"),
    ("FUNCTION", r"munus"),
    ("RETURN", r"redde"),
    ("TRUE", r"verus"),
    ("FALSE", r"falsus"),
    ("BREAK",r"abrumpe"),
    ("CONTINUE",r"continua"),
    ("PRINT", r"scribe"),
    ("ISEQUAL", r"par_est"),
    ("ISNOTEQUAL", r"par_non_est"),
    ("ISGREATER", r"maior_est"),
    ("ISLESS",r"minor_est")
]
