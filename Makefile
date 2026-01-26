CXX = g++
CC  = gcc

CXXFLAGS = -std=c++20 -Wall -Wextra -Iincludes
CFLAGS   = -std=c11  -Wall -Wextra -Iincludes

OBJS = \
  encoder/block/block.o \
  encoder/blockizer/blockizer.o \
  encoder/encoder.o \
  main.o

all: bitgrain

bitgrain: $(OBJS)
	$(CXX) $(OBJS) -o bitgrain

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c $< -o $@

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f $(OBJS) bitgrain
