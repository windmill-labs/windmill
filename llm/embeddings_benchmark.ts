const getWords = async () => {
  const response = await fetch("http://localhost:3001/searchData");
  const data: {
    asks: {
      summary: string;
      app: string;
    }[];
  } = await response.json();
  const words = data.asks
    .map((a) => [
      ...a.summary.split(" ").map((w: string) => w.toLowerCase()),
      ...a.app.split(" ").map((w: string) => w.toLowerCase()),
    ])
    .flat();
  return words;
};

const generateQuery = async (words: string[]) => {
  const word1 = words[Math.floor(Math.random() * words.length)];
  const word2 = words[Math.floor(Math.random() * words.length)];
  const word3 = words[Math.floor(Math.random() * words.length)];
  const query = `${word1} ${word2} ${word3}`;
  return query.substring(0, 3 + Math.random() * (query.length - 3));
};

async function sendRequest(q: string) {
  const time = Date.now();

  try {
    await fetch(
      "http://localhost:3001/scripts/query?" +
        new URLSearchParams({
          text: q,
        })
    );
    return {
      time: Date.now() - time,
      error: false,
    };
  } catch (err) {
    return {
      time: Date.now() - time,
      error: true,
    };
  }
}

async function benchmark() {
  // first fetch to mitigate cold start
  await fetch(
    "http://localhost:3001/scripts/query?" +
      new URLSearchParams({
        text: "init",
      })
  );
  const words = await getWords();
  const tryouts = [1, 10, 100, 1000, 10000];

  for (const tryout of tryouts) {
    const requests: Promise<{
      time: number;
      error: boolean;
    }>[] = [];
    for (let i = 0; i < tryout; i++) {
      const q = await generateQuery(words);
      requests.push(sendRequest(q));
    }
    const startTime = Date.now();
    const times = await Promise.all(requests);
    const duration = Date.now() - startTime;
    const avg = times.reduce((a, b) => a + b.time, 0) / times.length;
    const errors = times.filter((t) => t.error).length;
    console.log(`Average time for ${tryout} simultaneous requests: ${avg}ms`);
    console.log(
      `Total time for ${tryout} simultaneous requests: ${duration}ms`
    );
    console.log(`Number of errors: ${errors}`);
  }
}

benchmark();
