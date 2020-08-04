cd $(dirname "$0")
flatc --rust codec.fbs
if [ $? -eq 0 ]; then
	mv codec_generated.rs src/codec.rs
else
	exit 1
fi