# Nete Notes Extension System

Nete Notes supports a powerful extension system that allows developers to customize and enhance the application. Extensions can provide custom themes, add commands to the command bar, and create slash menu items for the editor.

## Table of Contents

- [Quick Start](#quick-start)
- [Extension Structure](#extension-structure)
- [Extension Manifest](#extension-manifest)
- [Themes](#themes)
- [Command Bar Commands](#command-bar-commands)
- [Slash Commands](#slash-commands)
- [Action Types](#action-types)
- [Examples](#examples)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Quick Start

1. Create a directory for your extension in the extensions folder:
   - **Linux**: `~/.config/Nete/extensions/my-extension/`
   - **macOS**: `~/Library/Application Support/Nete/extensions/my-extension/`
   - **Windows**: `%APPDATA%\Nete\extensions\my-extension\`

2. Create an `extension.toml` file in your extension directory

3. Restart Nete Notes - your extension will be loaded automatically

## Extension Structure

A minimal extension requires only a manifest file:

```
my-extension/
└── extension.toml          # Required: Extension manifest
```

A complete extension with themes:

```
my-extension/
├── extension.toml          # Required: Extension manifest
└── theme.css              # Optional: Custom theme styles
```

## Extension Manifest

The `extension.toml` file defines your extension's metadata, commands, and configuration.

### Basic Structure

```toml
[extension]
id = "my-extension"           # Required: Unique identifier (use kebab-case)
name = "My Extension"          # Required: Display name
version = "1.0.0"              # Required: Semantic version
author = "Your Name"           # Optional: Author name
description = "What it does"   # Optional: Short description

# Optional theme configuration
[theme]
css_file = "theme.css"

# Optional command bar commands
[[commands]]
id = "command-id"
label = "Command Label"
icon = "icon-name-symbolic"
action = "insert_text"
text = "Text to insert"

# Optional slash commands
[[slash_commands]]
id = "slash-id"
label = "Slash Label"
action = "insert_text"
text = "Text to insert"
aliases = ["alias1", "alias2"]
```

### Manifest Fields

#### `[extension]` - Metadata Section

| Field | Required | Description |
|-------|----------|-------------|
| `id` | Yes | Unique identifier for your extension. Use lowercase letters, numbers, and hyphens only |
| `name` | Yes | Human-readable name displayed in the UI |
| `version` | Yes | Version number (semantic versioning recommended) |
| `author` | No | Your name or organization |
| `description` | No | Brief description of what your extension does |

#### `[theme]` - Theme Section (Optional)

| Field | Required | Description |
|-------|----------|-------------|
| `css_file` | Yes | Path to CSS file, relative to extension directory |

#### `[[commands]]` - Command Bar Commands (Optional)

Repeat this section for each command you want to add to the command bar (Ctrl+K).

| Field | Required | Description |
|-------|----------|-------------|
| `id` | Yes | Unique identifier for this command |
| `label` | Yes | Display text in the command bar |
| `icon` | No | GTK icon name (default: `application-x-addon-symbolic`) |
| `action` | Yes | Action type (see [Action Types](#action-types)) |
| `text` | Depends | Text parameter for the action (required by some actions) |
| `shortcut` | No | Keyboard shortcut (future feature) |

#### `[[slash_commands]]` - Slash Commands (Optional)

Repeat this section for each slash command (triggered by typing `/` in the editor).

| Field | Required | Description |
|-------|----------|-------------|
| `id` | Yes | Unique identifier for this command |
| `label` | Yes | Display text in the slash menu |
| `action` | Yes | Action type (see [Action Types](#action-types)) |
| `text` | Depends | Text parameter for the action |
| `aliases` | No | Array of alternative names to match |

## Themes

Extensions can customize the appearance of Nete Notes using GTK4 CSS.

### Creating a Theme

1. Add the `[theme]` section to your `extension.toml`
2. Create a CSS file in your extension directory
3. Use GTK4 CSS syntax with Libadwaita variables

### Example theme.css

```css
/* Custom accent color */
@define-color accent_color @green_3;
@define-color accent_bg_color @green_4;

/* Editor styling */
textview {
    font-family: 'JetBrains Mono', monospace;
    font-size: 14px;
    line-height: 1.6;
}

/* Selection color */
textview selection {
    background-color: alpha(@accent_color, 0.3);
}

/* Sidebar styling */
.navigation-sidebar {
    background-color: alpha(@window_bg_color, 0.95);
}

/* Custom padding for notes list */
.navigation-sidebar row {
    padding: 12px;
    border-radius: 8px;
    margin: 2px 6px;
}

/* Hover effects */
.navigation-sidebar row:hover {
    background-color: alpha(@accent_color, 0.1);
}

/* Command palette styling */
.command-palette {
    border-radius: 16px;
    box-shadow: 0 8px 32px alpha(black, 0.3);
}
```

### Useful CSS Selectors

| Selector | Targets |
|----------|---------|
| `textview` | The main editor |
| `textview text` | The text content area |
| `textview selection` | Selected text |
| `.navigation-sidebar` | The notes list sidebar |
| `.command-palette` | The command bar overlay |
| `headerbar` | The window header |
| `.card` | Card-style containers |

### Libadwaita Color Variables

```css
@window_bg_color        /* Window background */
@view_bg_color          /* Content area background */
@headerbar_bg_color     /* Header bar background */
@card_bg_color          /* Card backgrounds */
@popover_bg_color       /* Popover/menu backgrounds */
@accent_color           /* Primary accent color */
@accent_bg_color        /* Accent background */
@destructive_color      /* Error/destructive color */
@success_color          /* Success color */
@warning_color          /* Warning color */
@borders                /* Border color */
```

## Command Bar Commands

Commands appear in the command palette (Ctrl+K) and can perform various actions.

### Example Commands

```toml
# Insert a template
[[commands]]
id = "insert-daily-note"
label = "Insert Daily Note Template"
icon = "calendar-symbolic"
action = "insert_text"
text = """# Daily Note - {{date}}

## Tasks
- [ ] 

## Notes

"""

# Quick formatting
[[commands]]
id = "insert-highlight"
label = "Insert Highlight Mark"
icon = "highlighter-symbolic"
action = "insert_text"
text = "==highlighted text=="

# Navigation command
[[commands]]
id = "open-todo-note"
label = "Open Todo List"
icon = "checkbox-checked-symbolic"
action = "open_note"
text = "Todo List"
```

## Slash Commands

Slash commands appear when typing `/` in the editor. They're perfect for quick insertions.

### Example Slash Commands

```toml
# Simple insertions
[[slash_commands]]
id = "heading"
label = "Heading 1"
action = "insert_text"
text = "# "
aliases = ["h1", "title"]

[[slash_commands]]
id = "heading2"
label = "Heading 2"
action = "insert_text"
text = "## "
aliases = ["h2", "subtitle"]

# Structured content
[[slash_commands]]
id = "table"
label = "Table"
action = "insert_text"
text = """| Column 1 | Column 2 |
|----------|----------|
|          |          |
"""
aliases = ["grid"]

# Note linking
[[slash_commands]]
id = "backlink"
label = "Backlink"
action = "insert_note_link"
text = "References"
aliases = ["link", "ref"]
```

## Action Types

Actions define what happens when a command is executed.

### `insert_text`

Inserts the specified text at the cursor position.

```toml
action = "insert_text"
text = "Text to insert"
```

### `insert_note_link`

Inserts a wiki-style link: `[[Note Title]]`

```toml
action = "insert_note_link"
text = "Note Title"  # The title inside the brackets
```

### `open_note`

Opens a note by title. If multiple notes match, the first one found is opened.

```toml
action = "open_note"
text = "Note Title"
```

### `external_command`

⚠️ **Not yet implemented** - Will execute external programs with user confirmation.

```toml
action = "external_command"
text = "/path/to/script.sh"
```

### `toggle_setting` / `set_setting`

⚠️ **Not yet implemented** - Will modify application settings.

## Examples

### Complete Productivity Extension

```toml
[extension]
id = "productivity-pack"
name = "Productivity Pack"
version = "1.0.0"
author = "Productive Developer"
description = "Templates and shortcuts for productivity workflows"

[[commands]]
id = "pomodoro-template"
label = "Insert Pomodoro Session"
icon = "timer-symbolic"
action = "insert_text"
text = """# Pomodoro Session

**Focus:** 
**Duration:** 25 minutes

## Notes

"""

[[commands]]
id = "meeting-notes"
label = "Meeting Notes Template"
icon = "system-users-symbolic"
action = "insert_text"
text = """# Meeting: {{topic}}
**Date:** {{date}}
**Attendees:** 

## Agenda

## Notes

## Action Items
- [ ] 
"""

[[slash_commands]]
id = "task"
label = "Task"
action = "insert_text"
text = "- [ ] "
aliases = ["todo", "checkbox"]

[[slash_commands]]
id = "priority"
label = "High Priority"
action = "insert_text"
text = "🔴 "
aliases = ["urgent", "important"]

[[slash_commands]]
id = "idea"
label = "Idea"
action = "insert_text"
text = "💡 "
aliases = ["lightbulb", "thought"]
```

### Complete Theme Extension

```toml
[extension]
id = "dark-pro-theme"
name = "Dark Pro"
version = "1.0.0"
author = "Theme Designer"
description = "A professional dark theme with enhanced contrast"

[theme]
css_file = "theme.css"
```

```css
/* theme.css */
@define-color accent_color #7aa2f7;
@define-color accent_bg_color #565f89;

/* Editor with enhanced readability */
textview {
    font-family: 'SF Mono', 'Fira Code', monospace;
    font-size: 15px;
    line-height: 1.7;
    color: #a9b1d6;
    background-color: #1a1b26;
}

textview selection {
    background-color: rgba(122, 162, 247, 0.3);
}

/* Sidebar with subtle styling */
.navigation-sidebar {
    background-color: #16161e;
    border-right: 1px solid #24283b;
}

.navigation-sidebar row:selected {
    background-color: rgba(122, 162, 247, 0.2);
    border-left: 3px solid @accent_color;
}

/* Cards and dialogs */
.card {
    background-color: #24283b;
    border-radius: 12px;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.2);
}
```

### Markdown Tools Extension

```toml
[extension]
id = "markdown-tools"
name = "Markdown Tools"
version = "1.0.0"
author = "Markdown Lover"
description = "Useful markdown formatting shortcuts"

[[commands]]
id = "insert-table"
label = "Insert Table (3x3)"
icon = "view-grid-symbolic"
action = "insert_text"
text = """| Header 1 | Header 2 | Header 3 |
|----------|----------|----------|
|          |          |          |
|          |          |          |
"""

[[commands]]
id = "insert-collapsible"
label = "Insert Collapsible Section"
icon = "view-reveal-symbolic"
action = "insert_text"
text = """<details>
<summary>Click to expand</summary>

Content here...

</details>
"""

[[slash_commands]]
id = "code"
label = "Code Block"
action = "insert_text"
text = "```\n\n```"
aliases = ["snippet", "source"]

[[slash_commands]]
id = "quote"
label = "Blockquote"
action = "insert_text"
text = "> "
aliases = ["cite", "quotation"]

[[slash_commands]]
id = "strikethrough"
label = "Strikethrough"
action = "insert_text"
text = "~~strikethrough~~"
aliases = ["strike", "delete"]
```

## Best Practices

### Extension IDs

- Use lowercase letters, numbers, and hyphens only
- Use a consistent prefix for your extensions (e.g., `acme-`)
- Keep it short but descriptive
- Examples: `quick-inserts`, `dark-theme-pro`, `acme-templates`

### Command Labels

- Use action-oriented language: "Insert...", "Open...", "Toggle..."
- Keep labels concise (under 30 characters)
- Use sentence case: "Insert timestamp" not "Insert Timestamp"

### Icons

Use standard GTK symbolic icons:
- `document-edit-symbolic` - Text editing
- `folder-symbolic` - Files/folders  
- `format-text-bold-symbolic` - Formatting
- `timer-symbolic` - Time-related
- `star-symbolic` - Favorites
- `tag-symbolic` - Tags/categories

Find more at: https://developer.gnome.org/hig/guidelines/app-icons.html

### Text Templates

- Use triple quotes (`"""`) for multi-line text
- Include placeholders like `{{date}}` or `{{title}}`
- Add helpful comments in templates

### Aliases

- Include common misspellings
- Add shorthand versions (e.g., "h1" for "Heading 1")
- Consider different naming conventions

### CSS Themes

- Test with both light and dark system themes
- Use alpha() for transparency instead of hardcoded colors
- Respect user preferences (font sizes, etc.)
- Test at different window sizes

## Troubleshooting

### Extension Not Loading

1. Check that `extension.toml` is valid TOML syntax
2. Verify the `[extension]` section has required fields (`id`, `name`, `version`)
3. Ensure the extension directory is in the correct location
4. Check file permissions (should be readable)
5. Restart Nete Notes completely

### Theme Not Applying

1. Verify `css_file` path in extension.toml is correct
2. Check that the CSS file exists and is readable
3. Test CSS syntax with a simple rule first
4. Check that your CSS selector targets exist

### Commands Not Appearing

1. Verify the extension is enabled (check `enabled.toml`)
2. Ensure action types are spelled correctly
3. Check that required `text` fields are present
4. Look for duplicate command IDs

### Debug Your Extension

Create a minimal test extension:

```toml
[extension]
id = "test-extension"
name = "Test Extension"
version = "1.0.0"

[[commands]]
id = "test-cmd"
label = "Test Command"
action = "insert_text"
text = "It works!"
```

If this loads, gradually add more complexity.

### Getting Help

- Check the example extension in the repository
- Look at other extensions for inspiration
- Report issues with the extension system

## Extension API Reference

### File Locations

| Platform | Extensions Directory |
|----------|---------------------|
| Linux | `~/.config/Nete/extensions/` |
| macOS | `~/Library/Application Support/Nete/extensions/` |
| Windows | `%APPDATA%\Nete\extensions\` |

### Extension Loading Order

1. Application starts
2. Base theme is applied
3. All extensions are discovered from the extensions directory
4. Enabled extensions are loaded
5. Extension themes are applied (in alphabetical order by extension ID)
6. Extension commands are registered
7. Extension slash commands are registered

### Enabled Extensions List

The file `enabled.toml` in the extensions directory controls which extensions are active:

```toml
extensions = [
    "example-extension",
    "my-custom-theme",
    "productivity-pack"
]
```

If this file doesn't exist, all discovered extensions are enabled by default.

---

Happy extending! 🚀
