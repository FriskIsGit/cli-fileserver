use std::fs::File;

// something that reads at an offset?
// maybe it's in the default lib
// TODO
struct FileFeeder {
    file: File,
    portion_size: usize
}

const MB_100: usize = 104_857_600;
