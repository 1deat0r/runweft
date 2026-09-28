import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { SCAFFOLD_STATUS } from '@runweft/protocol';
import './style.css';

function Inspector() {
  return <main>
    <p className="eyebrow">RUNWEFT / DEVELOPMENT SCAFFOLD</p>
    <h1>Work with a record.</h1>
    <p className="intro">A foundation for durable runs, explicit authority, and verified results.</p>
    <section aria-labelledby="status"><h2 id="status">Scaffold status</h2>
      <dl><dt>Phase</dt><dd>{SCAFFOLD_STATUS.phase}</dd><dt>Execution</dt><dd>Disabled</dd><dt>Coordinator connection</dt><dd>Not implemented</dd></dl>
      <p>This page displays build-time metadata. It is not connected to a running agent.</p>
    </section>
    <section aria-labelledby="next"><h2 id="next">Next milestone</h2><p>Define and verify the run protocol before implementing durable execution. Read <code>spec.md</code> and <code>issues/01-contracts.md</code> in the repository.</p></section>
    <footer>No provider calls, credentials, tool execution, or runtime permissions are available in this scaffold.</footer>
  </main>;
}
const root = document.getElementById('root');
if (!root) throw new Error('Missing application root');
createRoot(root).render(<StrictMode><Inspector /></StrictMode>);
