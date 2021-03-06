use crate::block::Block;

pub struct BlockChain {
    pub blocks: Vec<Block>,
}

impl BlockChain {
    /// add a new block if valid
    pub fn add(&mut self, new: Block) {
        if Block::is_valid_next_block(&new, &self.get_latest()) {
            // println!("added");
            self.blocks.push(new)
        }
        // println!("not added");
    }

    /// get new_blocks and if valid completely change the self.blocks
    pub fn replace(&mut self, new_blocks: Vec<Block>) {
        let new_chain = BlockChain { blocks: new_blocks };

        if new_chain.is_valid() && new_chain.blocks.len() > self.blocks.len() {
            self.blocks = new_chain.blocks;
        } else {
            // println!("not replace");
            // println!("{:?}", new_chain.blocks);
        }
    }

    /// return the latest block
    pub fn get_latest(&self) -> Block {
        self.blocks.last().unwrap().clone()
    }

    /// return the genesis block
    pub fn get_genesis() -> Block {
        Block {
            index: 0,
            previous_hash: String::from("0"),
            timestamp: 1465154705,
            data: String::from("my genesis block!!"),
            hash: String::from("816534932c2b7154836da6afc367695e6337db8a921823784c14378abed4f7d7"),
        }
    }

    /// check if the complete chain is valid
    fn is_valid(&self) -> bool {
        if *self.blocks.first().unwrap() != BlockChain::get_genesis() {
            // println!("genesis problem");
            return false;
        }

        let mut temp_blocks = vec![self.blocks.first().unwrap()];

        for i in 1..self.blocks.len() {
            if Block::is_valid_next_block(
                self.blocks.get(i).unwrap(),
                temp_blocks.get(i - 1).unwrap(),
            ) {
                temp_blocks.push(self.blocks.get(i).unwrap());
            } else {
                // println!("vaild problem");
                // println!("{}", i);
                return false;
            }
        }

        true
    }
}
