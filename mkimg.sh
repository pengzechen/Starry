rm disk.img
make disk_img
mkdir -p tmp
sudo mount disk.img tmp

# Copy guest OS binary image file.
sudo cp ../guest/nimbos/nimbos-aarch64-0.bin tmp/
# Copy guest dtb file.
sudo cp ../guest/dtb/nimbos-aarch64-0.dtb tmp/

# Copy guest OS binary image file.
sudo cp ../guest/nimbos/nimbos-aarch64-1.bin tmp/
# Copy guest dtb file.
sudo cp ../guest/dtb/nimbos-aarch64-1.dtb tmp/

sudo umount tmp