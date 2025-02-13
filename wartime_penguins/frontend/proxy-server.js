// proxy-server.js
import express from 'express';
import fetch from 'node-fetch';
const app = express();
const PORT = 3001;

app.use(express.json());

app.post('/issue_task', async (req, res) => {
  const { machineHash, fixedAddress, input } = req.body;
  try {
    const response = await fetch(
      `https://cartesi-coprocessor-solver-prod.fly.dev/issue_task/${machineHash}/${fixedAddress}`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ input })
      }
    );
    const data = await response.json();
    res.json(data);
  } catch (error) {
    res.status(500).json({ error: error.message });
  }
});

app.listen(PORT, () => {
  console.log(`Proxy server listening on port ${PORT}`);
});
