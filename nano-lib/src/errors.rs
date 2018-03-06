use super::block::BlockType;

error_chain!{
    errors {
        BlockReadError(type: BlockType, msg: String) {
            description("Error reading block into {:?}", type)
            display("Could not read block as {:?}: {}", type, msg)
        }
    }
}