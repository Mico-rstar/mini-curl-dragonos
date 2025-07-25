**1. https请求响应很慢，block时间过长**

**2. 不支持read_to_string**
    
    可能是因为dragonos默认socket行为是遇到eof直接关闭socket，而没有把eof信号返回给用户层, 而linux会把eof返回给用户层

**3. 响应结束读取判断还有问题**