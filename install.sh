# 已知bug
# 1. 添加时，中文输入不了，只能粘贴进去

echo 'export TODO_DIR="$HOME/.todo"
export TODO_FILE="$HOME/.todo/todo.txt"
export DONE_FILE="$HOME/.todo/done.txt"'>> ~/.bashrc
mkdir -p $HOME/.todo
touch $HOME/.todo/todo.txt
touch $HOME/.todo/done.txt