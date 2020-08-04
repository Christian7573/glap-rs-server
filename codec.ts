export const FromServer = { to_id: new Map(), from_id: new Map(), HandshakeAccepted:{},AddCelestialObject:{}, };
FromServer.to_id = new Map([ [FromServer.HandshakeAccepted,0],[FromServer.AddCelestialObject,1], ]);
FromServer.from_id = new Map([ [0,FromServer.HandshakeAccepted],[1,FromServer.AddCelestialObject], ]);

export const ToServer = { to_id: new Map(), from_id: new Map(), Handshake:{}, };
ToServer.to_id = new Map([ [ToServer.Handshake,0], ]);
ToServer.from_id = new Map([ [0,ToServer.Handshake], ]);

