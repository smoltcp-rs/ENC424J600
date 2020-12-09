tmux -2 new-session -d -s enc424j600 -n $USER
#              |
# 0:port-demux | 1:user
#              |
tmux split-window -h
tmux select-pane -t 0
tmux send-keys "rm itm.bin || true; run-itmdemux-follow" C-m
tmux select-pane -t 1
tmux send-keys "run-openocd-f4x; run-help" C-m
#    |
# 0: | 1:user
#    |------------
#    | 2:network
#    |
tmux split-window -v
tmux select-pane -t 2
tmux resize-pane -D 20

# Set default window
tmux select-window -t enc424j600:$USER

# Set focus on user pane
tmux select-pane -t 1

# Attach to session
tmux -2 attach-session -t enc424j600
