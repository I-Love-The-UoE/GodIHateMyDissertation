import type { ReactNode } from "react";
import { useState } from "react";
import { useStore } from "@/app/services";
import { Button } from "@/components/button";
import { Dialog } from "@/components/dialog";
import { spawn } from "@/lib/async";

export function CredentialDialog(): ReactNode {
  const prompt = useStore((state) => state.credentialPrompt);
  const setCredentialPrompt = useStore((state) => state.setCredentialPrompt);
  const storeCredential = useStore((state) => state.storeCredential);
  const [username, setUsername] = useState("");
  const [secret, setSecret] = useState("");

  if (prompt === undefined) {
    return null;
  }

  const submit = async (): Promise<void> => {
    if (await storeCredential(prompt.host, username.trim(), secret)) {
      setCredentialPrompt(undefined);
      setSecret("");
      prompt.retry();
    }
  };

  return (
    <Dialog
      open
      title={`Credentials for ${prompt.host}`}
      onClose={() => setCredentialPrompt(undefined)}
      footer={
        <>
          <Button onClick={() => setCredentialPrompt(undefined)}>Cancel</Button>
          <Button
            variant="primary"
            disabled={username.trim() === "" || secret === ""}
            onClick={() => spawn(submit())}
          >
            Save and retry
          </Button>
        </>
      }
    >
      <p className="hint">
        Stored in the operating system keychain. For GitHub and GitLab use a personal access token
        as the password.
      </p>
      <div className="field">
        <label htmlFor="cred-user">Username</label>
        <input
          id="cred-user"
          value={username}
          onChange={(event) => setUsername(event.target.value)}
        />
      </div>
      <div className="field">
        <label htmlFor="cred-secret">Password or token</label>
        <input
          id="cred-secret"
          type="password"
          value={secret}
          onChange={(event) => setSecret(event.target.value)}
        />
      </div>
    </Dialog>
  );
}
