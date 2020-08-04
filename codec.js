export const FromServer = { HandshakeAccepted:{},AddCelestialObject:{}, };
FromServer.to_id = new Map([ [FromServer.HandshakeAccepted,0],[FromServer.AddCelestialObject,1], ]);
FromServer.from_id = new Map([ [0,FromServer.HandshakeAccepted],[1,FromServer.AddCelestialObject], ]);

export const ToServer = { Handshake:{}, };
ToServer.to_id = new Map([ [ToServer.Handshake,0], ]);
ToServer.from_id = new Map([ [0,ToServer.Handshake], ]);

