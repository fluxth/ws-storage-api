import React from "react";
import { UserProps } from "../UserGenerator/logic";

// NOTE:
const promptEdit = (row: UserProps) => {
  const edited: { [key: string]: string } = {};
  for (let [key, value] of Object.entries(row)) {
    if (key === "id") {
      edited[key] = value;
      continue;
    }

    const result = prompt(`Edit: ${key}`, value);
    if (result === null) return null;
    edited[key] = result;
  }

  return edited as unknown as UserProps; // NOTE: Don't do this in prod
};

export const UserList: React.FC<{
  data: UserProps[];
  editUser: (id: string, row: UserProps) => void;
  deleteUser: (id: string) => void;
}> = ({ data, editUser, deleteUser }) => {
  return (
    <>
      <table>
        <thead>
          <tr>
            <th>ID</th>
            <th>Username</th>
            <th>Password</th>
            <th>Profile Image</th>
            <th>Joined Date</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {data.map((row) => (
            <tr>
              <td>{row.id}</td>
              <td>{row.username}</td>
              <td>{row.password}</td>
              <td>
                <code>{row.profile_image}</code>
              </td>
              <td>{new Date(row.joined_date).toLocaleString()}</td>
              <td>
                <button
                  onClick={() => {
                    const edited = promptEdit(row);
                    if (edited !== null) editUser(row.id, edited);
                  }}
                >
                  Edit
                </button>
                <button onClick={() => deleteUser(row.id)}>Delete</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </>
  );
};
