# GenericKit

## A Generic Toolkit

GenericKit is a generic development toolkit that provides a wide range of algorithms, data structures, 
functional components, scripting languages, and practical frameworks. It helps developers quickly build 
applications while avoiding the need to reinvent common solutions.

## Features

- **Algorithms**: Well-implemented algorithms for various use cases
- **Data Structures**: Efficient and reusable data structure implementations
- **Functional Components**: Modular functional programming utilities
- **Scripting Languages**: Support for multiple scripting languages
- **Practical Frameworks**: Ready-to-use development frameworks

## AI-Assisted Development

[AGENTS.md](AGENTS.md) is the entry point for AI coding assistants. It provides build commands, architecture overview,
and key conventions.

The `.agents/` directory contains AI development resources, including coding rules and task prompts:

```text
.agents/
├── rules/        # Coding standards (common + language-specific)
└── prompts/      # Task plans and system prompts
```

### Rules

The `rules/` subdirectory provides a layered rule system for AI coding assistants (e.g., Claude Code, Cursor).
It defines standards, conventions, and checklists to ensure consistent, high-quality code generation.
Language-specific rules take precedence over common rules (specific overrides general).

The rules are project-local and read directly by the AI assistant — no external installation needed.

## Getting Started

*(Add installation and usage instructions here)*

## License

The self-owned code of this project is licensed under the Apache License 2.0 and can be freely applied to commercial 
and non-commercial projects while retaining copyright information.
However, this project also uses some scattered open source code, please replace or remove it for commercial use.
Any commercial disputes or infringement caused by using this project have nothing to do with the project and developers 
and shall be at your own legal risk.
When using the code of this project, the license agreement should also indicate the license of the third-party libraries 
that this project depends on.
