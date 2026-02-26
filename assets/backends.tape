# tasuki backends demo: toggling backends on/off via config hot-reload
# Usage: vhs assets/backends.tape

Output assets/backends.gif

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
Type `export PATH="$HOME/.cargo/bin:$PATH"`
Enter
Sleep 200ms

# Seed demo data with both backends
Type "bash assets/demo-setup.sh"
Enter
Sleep 300ms

Type `(sleep 5 && sed -i '/\[backends.obsidian\]/,/^$/{s/enabled = true/enabled = false/}' ~/.config/tasuki/config.toml && sleep 5 && sed -i '/\[backends.obsidian\]/,/^$/{s/enabled = false/enabled = true/}' ~/.config/tasuki/config.toml && sleep 5 && sed -i '/\[backends.local\]/,/^$/{s/enabled = true/enabled = false/}' ~/.config/tasuki/config.toml && sleep 5 && sed -i '/\[backends.local\]/,/^$/{s/enabled = false/enabled = true/}' ~/.config/tasuki/config.toml) &`
Enter
Sleep 300ms
Type "clear"
Enter
Sleep 300ms
Show

# Launch TUI — both [local] and [obsidian] visible
Type "tasuki"
Sleep 300ms
Enter

# Both backends visible 
Sleep 1.5s
Down
Sleep 400ms
Down
Sleep 400ms
Down
Sleep 400ms
Tab
Sleep 500ms

# Obsidian disabled
Sleep 3s
Down
Sleep 400ms
Down
Sleep 400ms

# Obsidian enabled
Sleep 2s
Down
Sleep 400ms
Tab
Sleep 500ms
Down
Sleep 400ms

# Local disabled
Sleep 2.5s
Down
Sleep 400ms
Down
Sleep 400ms

# Local enabled
Sleep 2.5s

# Let viewer see everything restored
Sleep 1.5s
