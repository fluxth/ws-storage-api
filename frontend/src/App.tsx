import "./App.css";
import { useEffect, useState, useRef } from "react";

import { UserGenerator } from "./UserGenerator";
import { UserList } from "./UserList";
import { UserProps } from "./UserGenerator/logic";

enum Page {
  Generator,
  List,
}

export const App: React.FC = () => {
  const [page, setPage] = useState<Page>(Page.Generator);
  const [connected, setConnected] = useState<boolean>(false);
  const [data, setData] = useState<UserProps[]>([]);

  const ws = useRef<WebSocket | null>(null);

  useEffect(() => {
    // const uri = `ws://127.0.0.1:3030/user`; // DEV
    const uri = `ws://${location.host}/user`;
    ws.current = new WebSocket(uri);

    ws.current.onopen = () => {
      setConnected(true);
      if (ws.current) ws.current.send(JSON.stringify({ type: "get" }));
    };

    ws.current.onclose = () => {
      setConnected(false);
    };

    ws.current.onmessage = (msg) => {
      const payload = JSON.parse(msg.data);
      switch (payload.type) {
        case "reload":
          setData([...payload.data]);
          break;
        case "append":
          setData((old) => [...old, payload.data]);
          break;
        case "error":
          alert("Error: " + payload.message);
          break;
      }
    };

    return () => {
      if (ws.current) ws.current.close();
    };
  }, []);

  const addUser = (user: UserProps) => {
    if (ws.current)
      ws.current.send(JSON.stringify({ type: "add", data: user }));
  };

  const editUser = (id: string, user: UserProps) => {
    if (ws.current)
      ws.current.send(JSON.stringify({ type: "edit", id, data: user }));
  };

  const deleteUser = (id: string) => {
    if (ws.current) ws.current.send(JSON.stringify({ type: "delete", id }));
  };

  let component = null;
  switch (page) {
    case Page.Generator:
      component = (
        <>
          <div className="header">
            <button onClick={() => setPage(Page.List)}>List View</button>
          </div>
          <UserGenerator addUser={addUser} />
        </>
      );
      break;
    case Page.List:
      component = (
        <>
          <div className="header">
            <button onClick={() => setPage(Page.Generator)}>
              Generator View
            </button>
          </div>
          <UserList data={data} editUser={editUser} deleteUser={deleteUser} />
        </>
      );
      break;
  }

  return (
    <div className="App">
      <div className="status">
        WS: {connected === true ? "Connected" : "Disconnected"}
      </div>
      {component}
    </div>
  );
};

export default App;
