# tasuki hero demo: navigate, quick-add, edit, toggle done
# Usage: vhs assets/hero.tape

Output assets/hero.gif

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

# Seed demo data (hidden)
Hide
Type `export PATH="$HOME/.cargo/bin:$PATH"`
Enter
Sleep 200ms
Type "bash assets/demo-setup.sh && clear"
Enter
Sleep 500ms
Show

# Launch TUI
Type "tasuki"
Sleep 300ms
Enter
Sleep 1.5s

# Navigate down through a few tasks
Down
Sleep 350ms
Down
Sleep 350ms
Down
Sleep 350ms
Down
Sleep 350ms
Down
Sleep 350ms

# Jump to next group with Tab
Tab
Sleep 600ms
Tab
Sleep 600ms

# Collapse a group, then expand
Space
Sleep 700ms
Space
Sleep 600ms

# Quick-add a task with NLP
Type "a"
Sleep 500ms
Type@80ms "Ship API docs tomorrow (p1) #backend"
Sleep 800ms
Enter
Sleep 1.2s

# Navigate up to first task under first overdue group header
# After quick-add, cursor is still where it was. Go to top.
Up 20
Sleep 200ms
# Down once to get to first task (skip group header)
Down
Sleep 400ms

# Edit it — pre-populated with: Fix login redirect bug (p1) YYYY-MM-DD #backend
# Append #urgent tag
Type "e"
Sleep 600ms
Type@80ms " #urgent"
Sleep 800ms
Enter
Sleep 1.2s

# Toggle it done
Type "x"
Sleep 1.2s

# Quit
Type "q"
Sleep 500ms
