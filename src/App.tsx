import { CompletionStreamer } from "./components/organisms/CompletionStreamer";
import { WorktreeRunner } from "./components/organisms/WorktreeRunner";

function App() {
  return (
    <main style={{ padding: 24, fontFamily: "system-ui" }}>
      <h1>Hello, world</h1>
      <WorktreeRunner />
      <CompletionStreamer />
    </main>
  );
}

export default App;
