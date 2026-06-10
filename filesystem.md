  ### 1. Filesystem Implementation: fs.rs

  I created a flat directory layout with contiguous block allocation:

  • Block 0 (Superblock): Houses filesystem magic number ( 0x5349_4d50_4c45_4653 ), the next free block
  allocator index, and the current active file count.
  • Blocks 1–16 (Directory entries): Each block holds 8 directory entries of 64 bytes each, supporting a
  maximum of 128 flat files.
  • Blocks 17+ (Data Blocks): Contiguously stores actual file contents.
  • CRUD Operations: Implemented  format() ,  create_file() ,  read_file() ,  delete_file() , and
  list_files() .

  ### 2. Startup Mounting: main.rs

  Modified the startup sequence after NVMe initialization to try mounting the filesystem:

                nvme::init(device);
                match fs::read_superblock() {
                    Ok(sb) => {
                        let file_count = sb.file_count;
                        println!("FS: SimpleFS mounted successfully. Active files: {}", file_count);
                    }
                    Err(_) => {
                        println!("FS: SimpleFS is not formatted.");
                    }
                }

  ### 3. Interactive Shell Commands: shell.rs

  Registered 5 new commands inside the shell for interactive testing:

  •  fsformat : Formats the NVMe drive with SimpleFS.
  •  fsls : Lists all active files, including their size and starting block index.
  •  fswrite <filename> <content> : Writes the specified content to the file (automatically overwriting if
  the file already exists).
  •  fsread <filename> : Reads and prints the contents of a file to standard output.
  •  fsrm <filename> : Deletes the file from the filesystem by marking its entry slot as free.

  ### 4. Design Documentation

  A detailed description of the layout and structures can be found in the artifact design document:
  filesystem_design.md.
