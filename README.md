# Rust Terminal Editor

A simple terminal-based text editor written in Rust using **crossterm**.  
It supports **insert mode**, **command mode**, undo/redo, copy/paste, and file operations.  

---

## 🚀 Features

- ⌨️ Insert and command modes (Vim-like)
- 📝 Open, edit, and save text files
- ↩️ Undo / redo functionality
- 📋 Copy & paste within the editor
- 🔄 Cursor movement: arrows, Home/End support
- 🔒 Confirmation before exiting with unsaved changes
- Auto-handling of matching pairs for `()`, `{}`, `[]`, `"`, `'`

---

## 🧰 Requirements

- Rust toolchain (stable recommended)
- Linux, macOS, or Windows terminal
- `crossterm` crate (already included in `Cargo.toml`)

---

## 📦 Installation

1. Clone the repository:

```bash
git clone https://github.com/yourusername/rust-terminal-editor.git
cd rust-terminal-editor
