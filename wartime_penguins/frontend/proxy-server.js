import express from 'express';
import fetch from 'node-fetch';
import cors from 'cors';
const app = express();
const PORT = 3001;

app.use(express.json());
app.use(cors());

app.post('/issue_task', async (req, res) => {
  const { machineHash, fixedAddress, input } = req.body;
  try {
    const response = await fetch(
      `https://cartesi-coprocessor-solver-prod.fly.dev/issue_task/${machineHash}/${fixedAddress}`,
      {
        method: "POST",
        headers: { "Content-Type": "application/octet-stream" },
        body: Buffer.from(input.slice(2), "hex")
      }
    );
    const data = await response.json();
    res.json(data);
  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

app.get('/get_preimage/2/:bytes32', async (req, res) => {
    const { bytes32 } = req.params;
    try {
      const solverUrl = `https://cartesi-coprocessor-solver-prod.fly.dev/get_preimage/2/${bytes32}`;
      const response = await fetch(solverUrl);
      if (!response.ok) {
        const errText = await response.text();
        throw new Error(errText);
      }

      const arrayBuffer = await response.arrayBuffer();
      res.set('Content-Type', 'application/octet-stream');
      res.send(Buffer.from(arrayBuffer));
    } catch (error) {
      res.status(500).json({ error: error.message });
    }
  });

app.listen(PORT, () => {
  console.log(`Proxy server listening on port ${PORT}`);
});
