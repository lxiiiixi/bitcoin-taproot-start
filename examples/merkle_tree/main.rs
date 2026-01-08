mod utils;

use crate::utils::{MerkleProof, generate_proof, hash_pair, hash_single};
use hex::encode;

#[derive(Clone, Debug)]
enum MerkleNode {
    Leaf(Vec<u8>),
    Branch {
        left: Box<MerkleNode>,
        right: Box<MerkleNode>,
    },
}

impl MerkleNode {
    fn hash(&self) -> [u8; 32] {
        match self {
            MerkleNode::Leaf(data) => hash_single(data),
            MerkleNode::Branch { left, right } => {
                let left_hash = left.hash();
                let right_hash = right.hash();
                hash_pair(&left_hash, &right_hash)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct TaprootScript {
    // witness_version 固定为 0xc0 (OP_1)
    pub witness_version: u8,
    // 叶子脚本及其版本
    pub script_data: Vec<u8>,
    // 脚本版本（目前为 0xc0）
    pub leaf_version: u8,
}

#[derive(Clone, Debug)]
pub struct ScriptTree {
    // 树的节点
    pub root: MerkleNode,
    // 所有的脚本叶子
    pub leaves: Vec<TaprootScript>,
}

impl ScriptTree {
    // 从脚本列表构建脚本树
    pub fn build(scripts: Vec<TaprootScript>) -> Self {
        // 为了简化，假设脚本数量是 2 的幂次
        let mut leaves: Vec<MerkleNode> = scripts
            .iter()
            .map(|s| MerkleNode::Leaf(s.script_data.clone()))
            .collect();

        // 构建二叉树
        while leaves.len() > 1 {
            let mut new_level = Vec::new();
            for i in (0..leaves.len()).step_by(2) {
                if i + 1 < leaves.len() {
                    let left = leaves[i].clone();
                    let right = leaves[i + 1].clone();
                    new_level.push(MerkleNode::Branch {
                        left: Box::new(left),
                        right: Box::new(right),
                    });
                } else {
                    // 奇数个节点，复制最后一个
                    new_level.push(leaves[i].clone());
                }
            }
            leaves = new_level;
        }

        let root = leaves
            .into_iter()
            .next()
            .unwrap_or(MerkleNode::Leaf(vec![]));
        ScriptTree {
            root,
            leaves: scripts,
        }
    }

    pub fn root_hash(&self) -> [u8; 32] {
        self.root.hash()
    }
}

pub struct TaprootAddress {
    // 主公钥
    pub internal_key: [u8; 33],
    // 脚本树的根哈希
    pub script_tree_root: [u8; 32],
    // 最终的输出密钥 (internal_key + script_tree_root 相关)
    pub output_key: [u8; 33],
}

impl TaprootAddress {
    pub fn from_script_tree(internal_key: [u8; 33], script_tree: &ScriptTree) -> Self {
        let script_tree_root = script_tree.root_hash();
        // 简化：实际应用中输出密钥是通过 internal_key + tweak(script_tree_root) 计算的
        let mut output_key = internal_key;
        output_key[0] = 0x02; // 标记为 Taproot

        TaprootAddress {
            internal_key,
            script_tree_root,
            output_key,
        }
    }

    // 花费脚本控制的 UTXO 时，需要提供：
    // 1. 选择的脚本叶子
    // 2. Merkle 证明（从脚本到根）
    pub fn create_spend_witness(proof: &MerkleProof, signature: &[u8]) -> Vec<Vec<u8>> {
        let mut witness = vec![signature.to_vec()];

        // 加入脚本叶子
        witness.push(proof.leaf.clone());

        // 加入控制块 (control block)
        // 包含 leaf_version 和 merkle 路径信息
        let mut control = vec![0xc0]; // leaf_version = 0xc0

        for (sibling, is_right) in &proof.path {
            control.extend_from_slice(&sibling[..]);
        }

        witness.push(control);
        witness
    }
}

fn main() {
    // 创建 4 个脚本
    let scripts = vec![
        TaprootScript {
            witness_version: 0xc0,
            script_data: b"script_1".to_vec(),
            leaf_version: 0xc0,
        },
        TaprootScript {
            witness_version: 0xc0,
            script_data: b"script_2".to_vec(),
            leaf_version: 0xc0,
        },
        TaprootScript {
            witness_version: 0xc0,
            script_data: b"script_3".to_vec(),
            leaf_version: 0xc0,
        },
        TaprootScript {
            witness_version: 0xc0,
            script_data: b"script_4".to_vec(),
            leaf_version: 0xc0,
        },
    ];

    println!("1. 构建脚本树");
    let tree = ScriptTree::build(scripts);
    let root_hash = tree.root_hash();
    println!("   脚本树根哈希: {}\n", encode(&root_hash));

    println!("2. 生成第 2 个脚本的 Merkle 证明");
    if let Some(proof) = generate_proof(&tree, 1) {
        println!("   叶子数据: {:?}", String::from_utf8_lossy(&proof.leaf));
        println!(
            "   证明路径长度: {} (需要 {} 字节)",
            proof.path.len(),
            proof.proof_size()
        );

        println!("\n3. 验证 Merkle 证明");
        let valid = proof.verify(&root_hash);
        println!("   证明有效: {}\n", valid);

        println!("4. 构建 Taproot 地址");
        let internal_key = [0x02; 33]; // 简化的公钥
        let addr = TaprootAddress::from_script_tree(internal_key, &tree);
        println!("   脚本树根: {}", encode(&addr.script_tree_root));
        println!("   输出密钥: {}\n", encode(&addr.output_key));

        println!("5. 创建花费见证");
        let signature = b"signature_bytes";
        let witness = TaprootAddress::create_spend_witness(&proof, signature);
        println!("   见证元素数量: {}", witness.len());
        for (i, elem) in witness.iter().enumerate() {
            println!("     [{}] {} 字节", i, elem.len());
        }
    }
}
