FUNCTION_BLOCK OPENOT_EncodeMinimalMessage
VAR_INPUT
    SourceId : UDINT;
    RunId : ULINT;
    Seq : ULINT;
    SourceTime : ULINT;
    MessageTemplateId : UDINT;
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
    TotalLength := UINT#52,
    Flags := UINT#16#0008,
    SourceTime := SourceTime,
    RunId := RunId,
    Seq := Seq,
    SourceId := SourceId,
    EventTypeId := UDINT#16#0003,
    Buffer := Buffer);

Pos := OPENOT_WriteSlotHeader(
    Offset := Pos,
    Key := UINT#16#0014,
    TypeTag := BYTE#16#05,
    PayloadLength := BYTE#4,
    Buffer := Buffer);
Pos := OPENOT_WriteU32LE(Offset := Pos, Value := MessageTemplateId, Buffer := Buffer);

Crc := OPENOT_Crc32c(Data := Buffer, Length := Pos);
Pos := OPENOT_WriteDWordLE(Offset := Pos, Value := Crc, Buffer := Buffer);
Length := Pos;
END_FUNCTION_BLOCK
