import React, { useEffect } from "react";
import { UserProps, useUserGenerator } from "./logic";

export const UserGenerator: React.FC<{
  addUser: (user: UserProps) => void;
}> = ({ addUser }) => {
  const { generate, userinfo } = useUserGenerator();
  useEffect(() => {
    if (userinfo === null) return;
    addUser(userinfo);
  }, [userinfo]);

  return (
    <>
      <button style={{ fontSize: 24, marginBottom: 40 }} onClick={generate}>
        Generate & Save
      </button>
      <div>
        {userinfo && (
          <textarea rows={10} value={JSON.stringify(userinfo, null, "\t")} />
        )}
      </div>
    </>
  );
};
