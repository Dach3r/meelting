import { useState } from "react";
import { useCompletionStream } from "../../hooks/useCompletionStream";
import { useSecret } from "../../hooks/useSecret";

export function CompletionStreamer() {
  const [baseUrl, setBaseUrl] = useState("http://localhost:11434/v1");
  const [model, setModel] = useState("");
  const [prompt, setPrompt] = useState("");
  const [secretName, setSecretName] = useState("");
  const [apiKey, setApiKey] = useState("");
  const completion = useCompletionStream();
  const secret = useSecret();

  return (
    <section>
      <input
        placeholder="base url"
        value={baseUrl}
        onChange={(event) => setBaseUrl(event.target.value)}
      />
      <input
        placeholder="model"
        value={model}
        onChange={(event) => setModel(event.target.value)}
      />
      <input
        placeholder="keychain secret name (optional)"
        value={secretName}
        onChange={(event) => setSecretName(event.target.value)}
      />
      <input
        type="password"
        placeholder="api key"
        value={apiKey}
        onChange={(event) => setApiKey(event.target.value)}
      />
      <button
        onClick={async () => {
          await secret.save(secretName, apiKey);
          setApiKey("");
        }}
      >
        Save key
      </button>
      <span>{secret.status}</span>
      <br />
      <textarea
        placeholder="prompt"
        value={prompt}
        onChange={(event) => setPrompt(event.target.value)}
      />
      <button
        onClick={() =>
          completion.run({
            baseUrl,
            model,
            prompt,
            secretName: secretName || null,
          })
        }
      >
        Stream
      </button>
      <p>{completion.status}</p>
      <pre style={{ whiteSpace: "pre-wrap" }}>{completion.output}</pre>
    </section>
  );
}
