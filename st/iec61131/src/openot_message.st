FUNCTION_BLOCK OPENOT_EncodeMinimalMessage
VAR_INPUT
    SourceId : UDINT;
    Seq : ULINT;
END_VAR
VAR_OUTPUT
    Buffer : OPENOT_BYTE_BUFFER;
    Length : UINT;
END_VAR
VAR
    Pos : UINT;
    Crc : DWORD;
END_VAR

Pos := OPENOT_WriteHeader(
    TotalLength := UINT#44,
    Flags := UINT#16#0008,
    SourceTime := ULINT#1780000000000000000 + Seq,
    RunId := ULINT#1,
    Seq := Seq,
    SourceId := SourceId,
    EventTypeId := UDINT#16#0003,
    Buffer := Buffer);

Crc := OPENOT_Crc32c(Data := Buffer, Length := Pos);
Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := Crc, Buffer := Buffer);
Length := Pos;
END_FUNCTION_BLOCK
