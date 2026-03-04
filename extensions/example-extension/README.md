# Example Nete Notes Extension

This is a sample extension demonstrating the Nete Notes extension system.

## What's Included

This extension showcases all three types of extensibility:

1. **Themes** - Custom CSS styling (see `theme.css`)
2. **Command Bar Commands** - Actions accessible via Ctrl+K
3. **Slash Commands** - Quick insertions triggered by typing `/`

## Installation

### Option 1: Copy to Extensions Directory

**Linux:**
```bash
mkdir -p ~/.config/Nete/extensions/
cp -r example-extension ~/.config/Nete/extensions/
```

**macOS:**
```bash
mkdir -p ~/Library/Application\ Support/Nete/extensions/
cp -r example-extension ~/Library/Application\ Support/Nete/extensions/
```

**Windows:**
```powershell
# In PowerShell
Copy-Item -Recurse example-extension "$env:APPDATA\Nete\extensions\"
```

### Option 2: Symlink for Development

**Linux/macOS:**
```bash
ln -s /path/to/example-extension ~/.config/Nete/extensions/example-extension
```

## Commands Added

### Command Bar (Ctrl+K)

- **Insert Timestamp** - Inserts a calendar emoji 📅
- **Insert Todo Item** - Inserts a checkbox `- [ ] `
- **Insert Code Block** - Inserts markdown code fences
- **Insert Horizontal Rule** - Inserts `---`

### Slash Menu (type `/`)

- `/timestamp` or `/time` - Inserts ⏰
- `/checkbox` or `/todo` - Inserts `- [ ] `
- `/code` or `/snippet` - Inserts code block
- `/divider` or `/hr` - Inserts horizontal rule
- `/link` or `/ref` - Inserts wiki link `[[New Note]]`

## Customization

Edit `theme.css` to experiment with custom styling. Uncomment the example sections to see different customizations in action.

After editing `theme.css`, restart Nete Notes to see your changes.

## Creating Your Own Extension

1. Copy this folder and rename it
2. Update `extension.toml` with your extension's details
3. Modify or remove commands as needed
4. Edit `theme.css` or remove the `[theme]` section
5. Install your extension following the steps above

For full documentation, see `EXTENSIONS.md` in the main repository.

## License

This example is public domain. Use it as a starting point for your own extensions.
