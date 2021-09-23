_rireq_preexec() {
	rireq record "$1"
}

_rireq_history() {
	local exit_code=0
	tput smcup
	HISTORY="$(rireq history --print0 | fzf --read0 +s -q "$READLINE_LINE")"
	exit_code=$?
	tput rmcup
	if [ "$exit_code" -eq 0 ]; then
		READLINE_LINE=${HISTORY}
		READLINE_POINT=${#READLINE_LINE}
	fi
}

_rireq_setup() {
	local found=0
	for f in ${preexec_functions[*]}; do
		if [ "$f" = _rireq_preexec ]; then
			found=1
			break
		fi
	done
	case $found in
	0)	preexec_functions+=(_rireq_preexec) ;;
	esac

	bind -x '"\C-r": _rireq_history'
}

_rireq_setup
