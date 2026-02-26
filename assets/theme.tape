# tasuki theme demo: live theme switching
# Usage: vhs assets/theme.tape

Output assets/theme.gif

Set Shell "bash"
Set FontSize 16
Set Width 1100
Set Height 650
Set Padding 12
Set Framerate 24
Set TypingSpeed 60ms
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set WindowBarSize 30
Set BorderRadius 8
Set Margin 16
Set MarginFill "#11111b"

Hide
Type `export PATH="$HOME/.cargo/bin:$HOME/.local/share/omarchy/bin:$PATH"`
Enter
Sleep 200ms
Type `export OMARCHY_PATH="$HOME/.local/share/omarchy"`
Enter
Sleep 200ms

# Seed demo data, start on tokyo-night
Type "bash assets/demo-setup.sh"
Enter
Sleep 300ms
Type `sed -i 's/theme = "dark"/theme = "omarchy"/' ~/.config/tasuki/config.toml`
Enter
Sleep 200ms
Type "omarchy-theme-set tokyo-night 2>/dev/null"
Enter
Sleep 1s

# Schedule background theme switches:
Type `(sleep 2 && omarchy-theme-set rose-pine 2>/dev/null && sleep 2 && omarchy-theme-set catppuccin 2>/dev/null && sleep 2 && omarchy-theme-set miasma 2>/dev/null) &`
Enter
Sleep 300ms
Type "clear"
Enter
Sleep 300ms
Show

# Launch TUI
Type "tasuki"
Sleep 300ms
Enter
Sleep 2s

# Navigate while on tokyo-night
Down
Sleep 400ms
Down
Sleep 400ms
Tab
Sleep 600ms

Sleep 1s

# Navigate in rose-pine
Down
Sleep 400ms
Tab
Sleep 400ms
Down
Sleep 400ms
Tab
Sleep 600ms

Sleep 1s

# Navigate in catppuccin
Down
Sleep 400ms
Down
Sleep 400ms
Tab
Sleep 600ms

Sleep 1s

# Let viewer see miasma
Sleep 1.5s

