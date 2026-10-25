def slugify(text):
    """Lowercase ASCII words joined by single hyphens.

    Accents are dropped ("Ação" -> "acao"), anything that is not a letter or a digit separates
    words, and there is no hyphen at either end. slugify("  Olá, Mundo! 2026 ") == "ola-mundo-2026".
    """
    raise NotImplementedError
