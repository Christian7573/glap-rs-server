export const FromServer = { ZOutOfOrder:{},TestHandshake:{}, };
FromServer.to_id = new Map([ [FromServer.ZOutOfOrder,0],[FromServer.TestHandshake,1], ]);
FromServer.from_id = new Map([ [0,FromServer.ZOutOfOrder],[1,FromServer.TestHandshake], ]);

export const ToServer = { Handshake:{}, };
ToServer.to_id = new Map([ [ToServer.Handshake,0], ]);
ToServer.from_id = new Map([ [0,ToServer.Handshake], ]);

