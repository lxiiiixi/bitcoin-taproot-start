use bitcoin::{Amount, Transaction, Txid};
use serde_json::{Value, json};

/// Alchemy Client - 与 Bitcoin RPC 通信
pub struct AlchemyClient {
    endpoint: String,
    client: reqwest::Client,
}

/// UTXO 信息结构
#[derive(Clone, Debug)]
pub struct UtxoInfo {
    pub txid: String,
    pub vout: usize,
    pub value: u64,
    pub confirmations: Option<i64>,
}

/// gettxout 返回的脚本信息
#[derive(Clone, Debug)]
pub struct ScriptPubKey {
    pub asm: String,
    pub hex: String,
    pub address: Option<String>,
}

/// gettxout 返回的完整结果
#[derive(Clone, Debug)]
pub struct TxOut {
    pub bestblock: String,
    pub confirmations: i64,
    pub value: u64,
    pub script_pubkey: ScriptPubKey,
    pub coinbase: Option<bool>,
    pub txid: String,
    pub vout: u32,
}

impl AlchemyClient {
    /// 创建新的 AlchemyClient 实例
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// =====================================================
    /// 使用 gettxout 获取单个 UTXO 详情
    /// =====================================================
    ///
    /// 这是标准 Bitcoin RPC 方法
    /// 参数：
    ///   - txid: 交易 ID
    ///   - vout: 输出索引
    ///   - include_mempool: 是否包含 mempool 中的交易（默认 true）
    pub async fn get_tx_out(
        &self,
        txid: &str,
        vout: u32,
        include_mempool: bool,
    ) -> Result<Option<TxOut>, Box<dyn std::error::Error>> {
        println!(
            "  [RPC] 调用 gettxout (txid: {}..., vout: {})",
            &txid[..16],
            vout
        );

        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "gettxout",
            "params": [txid, vout, include_mempool]
        });

        let response = self
            .client
            .post(&self.endpoint)
            .json(&payload)
            .send()
            .await?;

        let result: Value = response.json().await?;

        println!("  [RPC] 响应: {:?}", result);

        // 检查错误
        if let Some(error) = result.get("error") {
            if !error.is_null() {
                let error_msg = error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");
                // gettxout 在 UTXO 已被花费时返回 null，这不是错误
                if error_msg.contains("spent") || error_msg.contains("not found") {
                    return Ok(None);
                }
                return Err(format!("RPC Error: {}", error_msg).into());
            }
        }

        // 如果结果是 null，表示 UTXO 已被花费或不存在
        if result["result"].is_null() {
            println!("  [RPC] 结果为 null，UTXO 已被花费或不存在");
            return Ok(None);
        }

        let res = &result["result"];

        // 解析返回结果
        let tx_out = TxOut {
            bestblock: res["bestblock"].as_str().unwrap_or("").to_string(),
            confirmations: res["confirmations"].as_i64().unwrap_or(0),
            value: Amount::from_btc(res["value"].as_f64().unwrap_or(0.0))?.to_sat(), // satoshis
            script_pubkey: ScriptPubKey {
                asm: res["scriptPubKey"]["asm"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                hex: res["scriptPubKey"]["hex"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                address: res["scriptPubKey"]["address"]
                    .as_str()
                    .map(|s| s.to_string()),
            },
            coinbase: res["coinbase"].as_bool(),
            txid: txid.to_string(),
            vout: vout,
        };

        Ok(Some(tx_out))
    }

    /// =====================================================
    /// 获取多个 UTXO 的详情
    /// =====================================================
    ///
    /// 使用 gettxout 批量获取多个 UTXO 的信息
    pub async fn get_multiple_tx_outs(
        &self,
        utxos: &[(&str, u32)],
    ) -> Result<Vec<Option<TxOut>>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        for (txid, vout) in utxos {
            let tx_out = self.get_tx_out(txid, *vout, true).await?;
            results.push(tx_out);
        }

        Ok(results)
    }

    /// =====================================================
    /// 广播交易
    /// =====================================================
    ///
    /// 使用 sendrawtransaction 将签名的交易广播到网络
    /// 参数：
    ///   - tx: 序列化的交易对象
    ///   - max_fee_rate: 最大费率（BTC/kB），0 表示不限制
    pub async fn broadcast_tx(
        &self,
        tx: &Transaction,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.broadcast_tx_hex(&bitcoin::consensus::encode::serialize_hex(tx), 0.1)
            .await
    }

    /// 使用 16 进制字符串广播交易
    pub async fn broadcast_tx_hex(
        &self,
        tx_hex: &str,
        max_fee_rate: f64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        println!("  [RPC] 调用 sendrawtransaction");

        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendrawtransaction",
            "params": [tx_hex, max_fee_rate]
        });

        let response = self
            .client
            .post(&self.endpoint)
            .json(&payload)
            .send()
            .await?;

        let result: Value = response.json().await?;

        // 检查错误
        if let Some(error) = result.get("error") {
            if !error.is_null() {
                let error_msg = error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");
                return Err(format!("Broadcast failed: {}", error_msg).into());
            }
        }

        // 返回 TXID
        if let Some(txid) = result["result"].as_str() {
            Ok(txid.to_string())
        } else {
            Err("Unknown broadcast error".into())
        }
    }

    /// =====================================================
    /// 辅助方法：验证 UTXO
    /// =====================================================
    ///
    /// 检查 UTXO 是否仍然可用
    pub async fn verify_utxo(
        &self,
        txid: &str,
        vout: u32,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match self.get_tx_out(txid, vout, true).await? {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }
}
