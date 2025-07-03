
build_thing: git_deps
	cmake -B bin -DPICO_SDK_PATH=../pico-sdk -DCMAKE_BUILD_TYPE=Debug
	make -j8 -C bin
# then manually copy bin/thing.uf2 to the pico

git_deps:
	cd ../pico-sdk && git submodule update --init lib/cyw43-driver lib/lwip

clean:
	rm -rf bin/*
